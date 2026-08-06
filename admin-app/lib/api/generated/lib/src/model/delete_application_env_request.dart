//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'delete_application_env_request.g.dart';

/// DeleteApplicationEnvRequest
///
/// Properties:
/// * [confirmFileName]
/// * [expectedVersion]
@BuiltValue()
abstract class DeleteApplicationEnvRequest implements Built<DeleteApplicationEnvRequest, DeleteApplicationEnvRequestBuilder> {
  @BuiltValueField(wireName: r'confirm_file_name')
  String get confirmFileName;

  @BuiltValueField(wireName: r'expected_version')
  int get expectedVersion;

  DeleteApplicationEnvRequest._();

  factory DeleteApplicationEnvRequest([void updates(DeleteApplicationEnvRequestBuilder b)]) = _$DeleteApplicationEnvRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(DeleteApplicationEnvRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<DeleteApplicationEnvRequest> get serializer => _$DeleteApplicationEnvRequestSerializer();
}

class _$DeleteApplicationEnvRequestSerializer implements PrimitiveSerializer<DeleteApplicationEnvRequest> {
  @override
  final Iterable<Type> types = const [DeleteApplicationEnvRequest, _$DeleteApplicationEnvRequest];

  @override
  final String wireName = r'DeleteApplicationEnvRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    DeleteApplicationEnvRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'confirm_file_name';
    yield serializers.serialize(
      object.confirmFileName,
      specifiedType: const FullType(String),
    );
    yield r'expected_version';
    yield serializers.serialize(
      object.expectedVersion,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    DeleteApplicationEnvRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required DeleteApplicationEnvRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'confirm_file_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.confirmFileName = valueDes;
          break;
        case r'expected_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.expectedVersion = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  DeleteApplicationEnvRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = DeleteApplicationEnvRequestBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
