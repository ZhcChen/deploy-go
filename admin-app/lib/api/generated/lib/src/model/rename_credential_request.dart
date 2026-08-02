//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'rename_credential_request.g.dart';

/// RenameCredentialRequest
///
/// Properties:
/// * [name]
/// * [version]
@BuiltValue()
abstract class RenameCredentialRequest implements Built<RenameCredentialRequest, RenameCredentialRequestBuilder> {
  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'version')
  int get version;

  RenameCredentialRequest._();

  factory RenameCredentialRequest([void updates(RenameCredentialRequestBuilder b)]) = _$RenameCredentialRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RenameCredentialRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RenameCredentialRequest> get serializer => _$RenameCredentialRequestSerializer();
}

class _$RenameCredentialRequestSerializer implements PrimitiveSerializer<RenameCredentialRequest> {
  @override
  final Iterable<Type> types = const [RenameCredentialRequest, _$RenameCredentialRequest];

  @override
  final String wireName = r'RenameCredentialRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RenameCredentialRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    yield r'version';
    yield serializers.serialize(
      object.version,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RenameCredentialRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RenameCredentialRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.version = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RenameCredentialRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RenameCredentialRequestBuilder();
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
