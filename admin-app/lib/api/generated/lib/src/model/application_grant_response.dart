//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_grant_response.g.dart';

/// ApplicationGrantResponse
///
/// Properties:
/// * [applicationId]
/// * [grantedAt]
@BuiltValue()
abstract class ApplicationGrantResponse implements Built<ApplicationGrantResponse, ApplicationGrantResponseBuilder> {
  @BuiltValueField(wireName: r'application_id')
  String get applicationId;

  @BuiltValueField(wireName: r'granted_at')
  String get grantedAt;

  ApplicationGrantResponse._();

  factory ApplicationGrantResponse([void updates(ApplicationGrantResponseBuilder b)]) = _$ApplicationGrantResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationGrantResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationGrantResponse> get serializer => _$ApplicationGrantResponseSerializer();
}

class _$ApplicationGrantResponseSerializer implements PrimitiveSerializer<ApplicationGrantResponse> {
  @override
  final Iterable<Type> types = const [ApplicationGrantResponse, _$ApplicationGrantResponse];

  @override
  final String wireName = r'ApplicationGrantResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationGrantResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'application_id';
    yield serializers.serialize(
      object.applicationId,
      specifiedType: const FullType(String),
    );
    yield r'granted_at';
    yield serializers.serialize(
      object.grantedAt,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ApplicationGrantResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationGrantResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'application_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.applicationId = valueDes;
          break;
        case r'granted_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.grantedAt = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ApplicationGrantResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationGrantResponseBuilder();
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
